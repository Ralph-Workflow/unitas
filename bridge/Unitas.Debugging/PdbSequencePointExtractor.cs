using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace Unitas.Debugging
{
    public static class PdbSequencePointExtractor
    {
        public static SequencePointTable ExtractFromAssembly(
            string assemblyPath,
            IReadOnlyList<string>? fallbackSourceFiles = null)
        {
            if (string.IsNullOrEmpty(assemblyPath))
            {
                throw new ArgumentException("Assembly path must be non-empty.", nameof(assemblyPath));
            }

            string fullAssemblyPath = Path.GetFullPath(assemblyPath);
            if (!File.Exists(fullAssemblyPath))
            {
                throw new FileNotFoundException("Assembly file not found.", fullAssemblyPath);
            }

            byte[] peBytes = File.ReadAllBytes(fullAssemblyPath);
            List<DiscoveredMethodRecord> discoveredMethods = ParseMethodsFromPeMetadata(peBytes);

            SequencePointTable table = new SequencePointTable();
            int nextId = 1;

            if (fallbackSourceFiles != null)
            {
                for (int f = 0; f < fallbackSourceFiles.Count; f++)
                {
                    string sourcePath = fallbackSourceFiles[f];
                    if (string.IsNullOrEmpty(sourcePath) || !File.Exists(sourcePath))
                    {
                        continue;
                    }

                    string fullSourcePath = Path.GetFullPath(sourcePath);
                    string[] lines = File.ReadAllLines(fullSourcePath);
                    MapSourceLinesToCompiledMethods(fullSourcePath, lines, discoveredMethods, table, ref nextId);
                }
            }

            return table;
        }

        private static List<DiscoveredMethodRecord> ParseMethodsFromPeMetadata(byte[] peBytes)
        {
            List<DiscoveredMethodRecord> result = new List<DiscoveredMethodRecord>();
            if (!TryFindMetadataStreams(
                peBytes,
                out int tablesOffset,
                out int tablesSize,
                out int stringsOffset,
                out int stringsSize))
            {
                return result;
            }

            _ = tablesSize;
            int pos = tablesOffset;
            if (pos + 24 > peBytes.Length)
            {
                return result;
            }

            byte heapSizes = peBytes[pos + 6];
            bool stringIndex4 = (heapSizes & 0x01) != 0;
            bool guidIndex4 = (heapSizes & 0x02) != 0;
            bool blobIndex4 = (heapSizes & 0x04) != 0;
            int stringIdxSize = stringIndex4 ? 4 : 2;
            int guidIdxSize = guidIndex4 ? 4 : 2;
            int blobIdxSize = blobIndex4 ? 4 : 2;

            ulong validMask = ReadUInt64(peBytes, pos + 8);
            pos += 24;

            int[] rowCounts = new int[64];
            for (int tableIndex = 0; tableIndex < 64; tableIndex++)
            {
                if ((validMask & (1UL << tableIndex)) != 0)
                {
                    if (pos + 4 > peBytes.Length)
                    {
                        return result;
                    }

                    rowCounts[tableIndex] = ReadInt32(peBytes, pos);
                    pos += 4;
                }
            }

            int typeDefRows = rowCounts[0x02];
            int fieldRows = rowCounts[0x04];
            int methodDefRows = rowCounts[0x06];
            if (typeDefRows <= 0)
            {
                return result;
            }

            int typeDefOrRefSize = ComputeCodedIndexSize(2, rowCounts[0x02], rowCounts[0x01], rowCounts[0x1B]);
            int fieldIdxSize = fieldRows >= 65536 ? 4 : 2;
            int methodIdxSize = methodDefRows >= 65536 ? 4 : 2;
            int paramIdxSize = rowCounts[0x08] >= 65536 ? 4 : 2;
            int resolutionScopeSize = ComputeCodedIndexSize(2, rowCounts[0x00], rowCounts[0x1A], rowCounts[0x23], rowCounts[0x01]);

            int moduleRowSize = 2 + stringIdxSize + (guidIdxSize * 3);
            int typeRefRowSize = resolutionScopeSize + (stringIdxSize * 2);
            int typeDefRowSize = 4 + (stringIdxSize * 2) + typeDefOrRefSize + fieldIdxSize + methodIdxSize;
            int fieldPtrRowSize = fieldIdxSize;
            int fieldRowSize = 2 + stringIdxSize + blobIdxSize;
            int methodPtrRowSize = methodIdxSize;
            int methodDefRowSize = 4 + 2 + 2 + stringIdxSize + blobIdxSize + paramIdxSize;

            int typeDefTableOffset = pos
                + (rowCounts[0x00] * moduleRowSize)
                + (rowCounts[0x01] * typeRefRowSize);
            int fieldTableOffset = typeDefTableOffset
                + (typeDefRows * typeDefRowSize)
                + (rowCounts[0x03] * fieldPtrRowSize);
            int methodDefTableOffset = fieldTableOffset
                + (fieldRows * fieldRowSize)
                + (rowCounts[0x05] * methodPtrRowSize);

            if (methodDefTableOffset + (methodDefRows * methodDefRowSize) > peBytes.Length)
            {
                return result;
            }

            RawTypeDef[] rawTypes = new RawTypeDef[typeDefRows + 1];
            for (int i = 1; i <= typeDefRows; i++)
            {
                int rowPos = typeDefTableOffset + ((i - 1) * typeDefRowSize);
                int nameIdx = ReadIndex(peBytes, rowPos + 4, stringIdxSize);
                int nsIdx = ReadIndex(peBytes, rowPos + 4 + stringIdxSize, stringIdxSize);
                int fieldListIdx = ReadIndex(peBytes, rowPos + 4 + (stringIdxSize * 2) + typeDefOrRefSize, fieldIdxSize);
                int methodListIdx = ReadIndex(peBytes, rowPos + 4 + (stringIdxSize * 2) + typeDefOrRefSize + fieldIdxSize, methodIdxSize);

                string typeName = ReadUtf8NullTerminated(peBytes, stringsOffset, stringsSize, nameIdx);
                string typeNs = ReadUtf8NullTerminated(peBytes, stringsOffset, stringsSize, nsIdx);
                rawTypes[i] = new RawTypeDef(typeNs, typeName, fieldListIdx, methodListIdx);
            }

            string[] fieldNames = new string[fieldRows + 1];
            for (int i = 1; i <= fieldRows; i++)
            {
                int rowPos = fieldTableOffset + ((i - 1) * fieldRowSize);
                int nameIdx = ReadIndex(peBytes, rowPos + 2, stringIdxSize);
                fieldNames[i] = ReadUtf8NullTerminated(peBytes, stringsOffset, stringsSize, nameIdx);
            }

            string[] methodNames = new string[methodDefRows + 1];
            for (int i = 1; i <= methodDefRows; i++)
            {
                int rowPos = methodDefTableOffset + ((i - 1) * methodDefRowSize);
                int nameIdx = ReadIndex(peBytes, rowPos + 8, stringIdxSize);
                methodNames[i] = ReadUtf8NullTerminated(peBytes, stringsOffset, stringsSize, nameIdx);
            }

            int[] enclosingTypeByNested = TryReadNestedClassMap(
                peBytes,
                methodDefTableOffset + (methodDefRows * methodDefRowSize),
                rowCounts,
                stringIdxSize,
                blobIdxSize,
                guidIdxSize);

            for (int t = 1; t <= typeDefRows; t++)
            {
                RawTypeDef rawType = rawTypes[t];
                int fieldStart = Math.Max(1, rawType.FieldList);
                int fieldEnd = t < typeDefRows ? rawTypes[t + 1].FieldList : fieldRows + 1;
                int methodStart = Math.Max(1, rawType.MethodList);
                int methodEnd = t < typeDefRows ? rawTypes[t + 1].MethodList : methodDefRows + 1;

                List<string> hoistedLocals = new List<string>();
                for (int f = fieldStart; f < fieldEnd && f <= fieldRows; f++)
                {
                    string rawFieldName = fieldNames[f];
                    if (TryParseHoistedLocalName(rawFieldName, out string hoistedName))
                    {
                        if (!hoistedLocals.Contains(hoistedName))
                        {
                            hoistedLocals.Add(hoistedName);
                        }
                    }
                    else if (!string.IsNullOrEmpty(rawFieldName)
                        && !rawFieldName.StartsWith("<", StringComparison.Ordinal)
                        && !rawFieldName.Contains("$", StringComparison.Ordinal))
                    {
                        if (!hoistedLocals.Contains(rawFieldName))
                        {
                            hoistedLocals.Add(rawFieldName);
                        }
                    }
                }

                bool isStateMachine = TryParseStateMachineMethodName(rawType.Name, out string originMethod);
                int enclosingIdx = t < enclosingTypeByNested.Length ? enclosingTypeByNested[t] : 0;

                if (isStateMachine)
                {
                    bool hasMoveNext = false;
                    for (int m = methodStart; m < methodEnd && m <= methodDefRows; m++)
                    {
                        if (string.Equals(methodNames[m], "MoveNext", StringComparison.Ordinal))
                        {
                            hasMoveNext = true;
                            break;
                        }
                    }

                    if (hasMoveNext)
                    {
                        string declaringFull = enclosingIdx > 0 && enclosingIdx <= typeDefRows
                            ? BuildFullTypeName(enclosingIdx, rawTypes, enclosingTypeByNested)
                            : FindFallbackEnclosingFullName(t, rawTypes);
                        string display = string.IsNullOrEmpty(declaringFull)
                            ? originMethod
                            : declaringFull + "." + originMethod;
                        result.Add(new DiscoveredMethodRecord(display, originMethod, hoistedLocals, true));
                    }

                    continue;
                }

                string fullTypeName = BuildFullTypeName(t, rawTypes, enclosingTypeByNested);
                for (int m = methodStart; m < methodEnd && m <= methodDefRows; m++)
                {
                    string methodName = methodNames[m];
                    if (string.IsNullOrEmpty(methodName)
                        || string.Equals(methodName, ".ctor", StringComparison.Ordinal)
                        || string.Equals(methodName, ".cctor", StringComparison.Ordinal))
                    {
                        continue;
                    }

                    string display = string.IsNullOrEmpty(fullTypeName)
                        ? methodName
                        : fullTypeName + "." + methodName;
                    result.Add(new DiscoveredMethodRecord(display, methodName, hoistedLocals, false));
                }
            }

            return result;
        }

        private static string FindFallbackEnclosingFullName(int stateMachineTypeIdx, RawTypeDef[] rawTypes)
        {
            for (int candidate = stateMachineTypeIdx - 1; candidate >= 1; candidate--)
            {
                string name = rawTypes[candidate].Name;
                if (!string.IsNullOrEmpty(name)
                    && !name.StartsWith("<", StringComparison.Ordinal))
                {
                    string ns = rawTypes[candidate].Namespace;
                    return string.IsNullOrEmpty(ns) ? name : ns + "." + name;
                }
            }

            return string.Empty;
        }

        private static string BuildFullTypeName(int typeIdx, RawTypeDef[] rawTypes, int[] enclosingByNested)
        {
            if (typeIdx <= 0 || typeIdx >= rawTypes.Length)
            {
                return string.Empty;
            }

            int enclosing = typeIdx < enclosingByNested.Length ? enclosingByNested[typeIdx] : 0;
            if (enclosing > 0 && enclosing != typeIdx)
            {
                string parentFull = BuildFullTypeName(enclosing, rawTypes, enclosingByNested);
                return string.IsNullOrEmpty(parentFull)
                    ? rawTypes[typeIdx].Name
                    : parentFull + "." + rawTypes[typeIdx].Name;
            }

            string ns = rawTypes[typeIdx].Namespace;
            string name = rawTypes[typeIdx].Name;
            if (string.Equals(name, "<Module>", StringComparison.Ordinal))
            {
                return string.Empty;
            }

            return string.IsNullOrEmpty(ns) ? name : ns + "." + name;
        }

        private static int[] TryReadNestedClassMap(
            byte[] peBytes,
            int afterMethodDefOffset,
            int[] rowCounts,
            int stringIdxSize,
            int blobIdxSize,
            int guidIdxSize)
        {
            int typeDefRows = rowCounts[0x02];
            int[] map = new int[typeDefRows + 1];
            int nestedRows = rowCounts[0x29];
            if (nestedRows <= 0)
            {
                return map;
            }

            int typeDefIdxSize = typeDefRows >= 65536 ? 4 : 2;
            int nestedRowSize = typeDefIdxSize * 2;

            _ = stringIdxSize;
            _ = blobIdxSize;
            _ = guidIdxSize;
            _ = afterMethodDefOffset;
            _ = nestedRowSize;

            return map;
        }

        private static void MapSourceLinesToCompiledMethods(
            string fullSourcePath,
            string[] lines,
            List<DiscoveredMethodRecord> discoveredMethods,
            SequencePointTable table,
            ref int nextId)
        {
            string currentNamespace = string.Empty;
            string currentClass = string.Empty;
            string currentMethodDisplay = string.Empty;
            List<string> currentLocals = new List<string>();
            int methodBraceDepth = 0;
            bool insideMethod = false;

            for (int i = 0; i < lines.Length; i++)
            {
                int lineNumber = i + 1;
                string trimmed = lines[i].Trim();

                if (trimmed.StartsWith("namespace ", StringComparison.Ordinal))
                {
                    string nsCandidate = trimmed.Substring("namespace ".Length).Trim();
                    int braceIdx = nsCandidate.IndexOf('{');
                    if (braceIdx >= 0)
                    {
                        nsCandidate = nsCandidate.Substring(0, braceIdx).Trim();
                    }

                    currentNamespace = nsCandidate;
                    continue;
                }

                if (!insideMethod && TryExtractClassName(trimmed, out string parsedClass))
                {
                    currentClass = parsedClass;
                    continue;
                }

                if (!insideMethod)
                {
                    if (TryMatchMethodHeader(trimmed, currentNamespace, currentClass, discoveredMethods, out DiscoveredMethodRecord? matchedMethod))
                    {
                        insideMethod = true;
                        currentMethodDisplay = matchedMethod!.DisplayName;
                        currentLocals = new List<string>(matchedMethod.HoistedLocals);
                        ExtractParameterOrLocalNamesFromLine(trimmed, currentLocals);
                        methodBraceDepth = CountChar(trimmed, '{') - CountChar(trimmed, '}');
                    }

                    continue;
                }

                int openCount = CountChar(trimmed, '{');
                int closeCount = CountChar(trimmed, '}');
                methodBraceDepth += openCount - closeCount;

                ExtractParameterOrLocalNamesFromLine(trimmed, currentLocals);

                if (IsExecutableSourceLine(trimmed))
                {
                    table.Add(nextId++, fullSourcePath, lineNumber, currentMethodDisplay, new List<string>(currentLocals));
                }

                if (methodBraceDepth <= 0 && (openCount > 0 || closeCount > 0 || trimmed.EndsWith(";", StringComparison.Ordinal)))
                {
                    insideMethod = false;
                    currentMethodDisplay = string.Empty;
                    currentLocals.Clear();
                    methodBraceDepth = 0;
                }
            }
        }

        private static bool TryExtractClassName(string trimmedLine, out string className)
        {
            className = string.Empty;
            int classIdx = trimmedLine.IndexOf("class ", StringComparison.Ordinal);
            if (classIdx < 0)
            {
                return false;
            }

            if (classIdx > 0 && char.IsLetterOrDigit(trimmedLine[classIdx - 1]))
            {
                return false;
            }

            int start = classIdx + "class ".Length;
            while (start < trimmedLine.Length && char.IsWhiteSpace(trimmedLine[start]))
            {
                start++;
            }

            int end = start;
            while (end < trimmedLine.Length && (char.IsLetterOrDigit(trimmedLine[end]) || trimmedLine[end] == '_'))
            {
                end++;
            }

            if (end <= start)
            {
                return false;
            }

            className = trimmedLine.Substring(start, end - start);
            return true;
        }

        private static bool TryMatchMethodHeader(
            string trimmedLine,
            string currentNamespace,
            string currentClass,
            List<DiscoveredMethodRecord> discoveredMethods,
            out DiscoveredMethodRecord? matched)
        {
            matched = null;
            DiscoveredMethodRecord? fallbackMatch = null;
            string expectedPrefix = string.IsNullOrEmpty(currentNamespace)
                ? currentClass
                : (string.IsNullOrEmpty(currentClass) ? currentNamespace : currentNamespace + "." + currentClass);

            for (int i = 0; i < discoveredMethods.Count; i++)
            {
                DiscoveredMethodRecord candidate = discoveredMethods[i];
                if (string.IsNullOrEmpty(candidate.SimpleName) || !ContainsMethodHeader(trimmedLine, candidate.SimpleName))
                {
                    continue;
                }

                if (!string.IsNullOrEmpty(expectedPrefix)
                    && string.Equals(candidate.DisplayName, expectedPrefix + "." + candidate.SimpleName, StringComparison.Ordinal))
                {
                    if (candidate.IsStateMachine)
                    {
                        matched = candidate;
                        return true;
                    }

                    matched = candidate;
                }
                else if (fallbackMatch == null || candidate.IsStateMachine)
                {
                    fallbackMatch = candidate;
                }
            }

            if (matched != null)
            {
                return true;
            }

            if (fallbackMatch != null)
            {
                string fullDisplay = !string.IsNullOrEmpty(expectedPrefix)
                    ? expectedPrefix + "." + fallbackMatch.SimpleName
                    : fallbackMatch.DisplayName;
                matched = new DiscoveredMethodRecord(fullDisplay, fallbackMatch.SimpleName, fallbackMatch.HoistedLocals, fallbackMatch.IsStateMachine);
                return true;
            }

            return false;
        }

        private static void ExtractParameterOrLocalNamesFromLine(string trimmedLine, List<string> locals)
        {
            if (trimmedLine.StartsWith("for ", StringComparison.Ordinal) || trimmedLine.StartsWith("for(", StringComparison.Ordinal))
            {
                int openParen = trimmedLine.IndexOf('(');
                int firstSemi = openParen >= 0 ? trimmedLine.IndexOf(';', openParen + 1) : -1;
                if (openParen >= 0 && firstSemi > openParen)
                {
                    string initPart = trimmedLine.Substring(openParen + 1, firstSemi - openParen - 1).Trim();
                    TryExtractDeclaredIdentifier(initPart, locals);
                }

                return;
            }

            int eqIdx = trimmedLine.IndexOf('=');
            if (eqIdx > 0)
            {
                string leftSide = trimmedLine.Substring(0, eqIdx).Trim();
                TryExtractDeclaredIdentifier(leftSide, locals);
            }
        }

        private static void TryExtractDeclaredIdentifier(string declarationLeftSide, List<string> locals)
        {
            string[] tokens = declarationLeftSide.Split(new[] { ' ', '\t' }, StringSplitOptions.RemoveEmptyEntries);
            if (tokens.Length != 2)
            {
                return;
            }

            string typeToken = tokens[0];
            string nameToken = tokens[1];
            if (string.Equals(typeToken, "return", StringComparison.Ordinal)
                || string.Equals(typeToken, "yield", StringComparison.Ordinal)
                || string.Equals(typeToken, "if", StringComparison.Ordinal)
                || string.Equals(typeToken, "while", StringComparison.Ordinal))
            {
                return;
            }

            if (IsValidIdentifier(nameToken) && !locals.Contains(nameToken))
            {
                locals.Add(nameToken);
            }
        }

        private static bool IsValidIdentifier(string token)
        {
            if (string.IsNullOrEmpty(token) || (!char.IsLetter(token[0]) && token[0] != '_'))
            {
                return false;
            }

            for (int i = 1; i < token.Length; i++)
            {
                if (!char.IsLetterOrDigit(token[i]) && token[i] != '_')
                {
                    return false;
                }
            }

            return true;
        }

        private static bool ContainsMethodHeader(string trimmedLine, string methodName)
        {
            if (trimmedLine.StartsWith("return ", StringComparison.Ordinal)
                || trimmedLine.StartsWith("yield ", StringComparison.Ordinal)
                || trimmedLine.StartsWith("if ", StringComparison.Ordinal)
                || trimmedLine.StartsWith("if(", StringComparison.Ordinal)
                || trimmedLine.StartsWith("for ", StringComparison.Ordinal)
                || trimmedLine.StartsWith("for(", StringComparison.Ordinal)
                || trimmedLine.StartsWith("while ", StringComparison.Ordinal)
                || trimmedLine.StartsWith("while(", StringComparison.Ordinal))
            {
                return false;
            }

            if (trimmedLine.EndsWith(";", StringComparison.Ordinal) && !trimmedLine.Contains("=>", StringComparison.Ordinal))
            {
                return false;
            }

            int idx = trimmedLine.IndexOf(methodName, StringComparison.Ordinal);
            if (idx <= 0)
            {
                return false;
            }

            char prev = trimmedLine[idx - 1];
            if (!char.IsWhiteSpace(prev))
            {
                return false;
            }

            int after = idx + methodName.Length;
            while (after < trimmedLine.Length && char.IsWhiteSpace(trimmedLine[after]))
            {
                after++;
            }

            return after < trimmedLine.Length && trimmedLine[after] == '(';
        }

        private static bool IsExecutableSourceLine(string trimmed)
        {
            if (string.IsNullOrEmpty(trimmed)
                || string.Equals(trimmed, "{", StringComparison.Ordinal)
                || string.Equals(trimmed, "}", StringComparison.Ordinal))
            {
                return false;
            }

            return true;
        }

        private static int CountChar(string text, char target)
        {
            int count = 0;
            for (int i = 0; i < text.Length; i++)
            {
                if (text[i] == target)
                {
                    count++;
                }
            }

            return count;
        }

        private static bool TryParseStateMachineMethodName(string typeName, out string methodName)
        {
            methodName = string.Empty;
            if (!typeName.StartsWith("<", StringComparison.Ordinal))
            {
                return false;
            }

            int closeAngle = typeName.IndexOf(">d__", StringComparison.Ordinal);
            if (closeAngle <= 1)
            {
                return false;
            }

            methodName = typeName.Substring(1, closeAngle - 1);
            return true;
        }

        private static bool TryParseHoistedLocalName(string fieldName, out string localName)
        {
            localName = string.Empty;
            if (!fieldName.StartsWith("<", StringComparison.Ordinal))
            {
                return false;
            }

            int closeAngle = fieldName.IndexOf(">5__", StringComparison.Ordinal);
            if (closeAngle <= 1)
            {
                return false;
            }

            localName = fieldName.Substring(1, closeAngle - 1);
            return true;
        }

        private static bool TryFindMetadataStreams(
            byte[] pe,
            out int tablesOffset,
            out int tablesSize,
            out int stringsOffset,
            out int stringsSize)
        {
            tablesOffset = 0;
            tablesSize = 0;
            stringsOffset = 0;
            stringsSize = 0;

            if (pe.Length < 64 || pe[0] != (byte)'M' || pe[1] != (byte)'Z')
            {
                return false;
            }

            int peHeaderOffset = ReadInt32(pe, 0x3C);
            if (peHeaderOffset < 0 || peHeaderOffset + 24 > pe.Length)
            {
                return false;
            }

            if (pe[peHeaderOffset] != (byte)'P' || pe[peHeaderOffset + 1] != (byte)'E')
            {
                return false;
            }

            int numberOfSections = ReadUInt16(pe, peHeaderOffset + 6);
            int sizeOfOptionalHeader = ReadUInt16(pe, peHeaderOffset + 20);
            int optionalHeaderOffset = peHeaderOffset + 24;
            if (optionalHeaderOffset + sizeOfOptionalHeader > pe.Length)
            {
                return false;
            }

            int magic = ReadUInt16(pe, optionalHeaderOffset);
            int cliHeaderDataDirOffset = magic == 0x20B
                ? optionalHeaderOffset + 112 + (14 * 8)
                : optionalHeaderOffset + 96 + (14 * 8);

            if (cliHeaderDataDirOffset + 8 > pe.Length)
            {
                return false;
            }

            int cliHeaderRva = ReadInt32(pe, cliHeaderDataDirOffset);
            if (cliHeaderRva <= 0)
            {
                return false;
            }

            int sectionTableOffset = optionalHeaderOffset + sizeOfOptionalHeader;
            int cliHeaderFileOffset = RvaToFileOffset(pe, sectionTableOffset, numberOfSections, cliHeaderRva);
            if (cliHeaderFileOffset < 0 || cliHeaderFileOffset + 16 > pe.Length)
            {
                return false;
            }

            int metadataRva = ReadInt32(pe, cliHeaderFileOffset + 8);
            int metadataFileOffset = RvaToFileOffset(pe, sectionTableOffset, numberOfSections, metadataRva);
            if (metadataFileOffset < 0 || metadataFileOffset + 16 > pe.Length)
            {
                return false;
            }

            if (ReadInt32(pe, metadataFileOffset) != 0x424A5342)
            {
                return false;
            }

            int versionLength = ReadInt32(pe, metadataFileOffset + 12);
            int alignedVersion = (versionLength + 3) & ~3;
            int streamsHeaderOffset = metadataFileOffset + 16 + alignedVersion;
            if (streamsHeaderOffset + 4 > pe.Length)
            {
                return false;
            }

            int streamCount = ReadUInt16(pe, streamsHeaderOffset + 2);
            int cursor = streamsHeaderOffset + 4;

            for (int s = 0; s < streamCount; s++)
            {
                if (cursor + 8 > pe.Length)
                {
                    return false;
                }

                int streamRelOffset = ReadInt32(pe, cursor);
                int streamByteSize = ReadInt32(pe, cursor + 4);
                cursor += 8;

                int nameStart = cursor;
                while (cursor < pe.Length && pe[cursor] != 0)
                {
                    cursor++;
                }

                string streamName = Encoding.ASCII.GetString(pe, nameStart, cursor - nameStart);
                cursor++;
                while ((cursor - metadataFileOffset) % 4 != 0)
                {
                    cursor++;
                }

                if (string.Equals(streamName, "#~", StringComparison.Ordinal)
                    || string.Equals(streamName, "#-", StringComparison.Ordinal))
                {
                    tablesOffset = metadataFileOffset + streamRelOffset;
                    tablesSize = streamByteSize;
                }
                else if (string.Equals(streamName, "#Strings", StringComparison.Ordinal))
                {
                    stringsOffset = metadataFileOffset + streamRelOffset;
                    stringsSize = streamByteSize;
                }
            }

            return tablesOffset > 0 && stringsOffset > 0;
        }

        private static int RvaToFileOffset(byte[] pe, int sectionTableOffset, int numberOfSections, int rva)
        {
            for (int i = 0; i < numberOfSections; i++)
            {
                int secOffset = sectionTableOffset + (i * 40);
                if (secOffset + 40 > pe.Length)
                {
                    break;
                }

                int virtualSize = ReadInt32(pe, secOffset + 8);
                int virtualAddress = ReadInt32(pe, secOffset + 12);
                int sizeOfRawData = ReadInt32(pe, secOffset + 16);
                int pointerToRawData = ReadInt32(pe, secOffset + 20);
                int span = Math.Max(virtualSize, sizeOfRawData);

                if (rva >= virtualAddress && rva < virtualAddress + span)
                {
                    return pointerToRawData + (rva - virtualAddress);
                }
            }

            return -1;
        }

        private static int ComputeCodedIndexSize(int tagBits, params int[] tableRowCounts)
        {
            int maxRows = 0;
            for (int i = 0; i < tableRowCounts.Length; i++)
            {
                if (tableRowCounts[i] > maxRows)
                {
                    maxRows = tableRowCounts[i];
                }
            }

            int threshold = 1 << (16 - tagBits);
            return maxRows >= threshold ? 4 : 2;
        }

        private static int ReadIndex(byte[] buffer, int offset, int size)
        {
            return size == 4 ? ReadInt32(buffer, offset) : ReadUInt16(buffer, offset);
        }

        private static ushort ReadUInt16(byte[] buffer, int offset)
        {
            return (ushort)(buffer[offset] | (buffer[offset + 1] << 8));
        }

        private static int ReadInt32(byte[] buffer, int offset)
        {
            return buffer[offset]
                | (buffer[offset + 1] << 8)
                | (buffer[offset + 2] << 16)
                | (buffer[offset + 3] << 24);
        }

        private static ulong ReadUInt64(byte[] buffer, int offset)
        {
            uint lo = (uint)ReadInt32(buffer, offset);
            uint hi = (uint)ReadInt32(buffer, offset + 4);
            return lo | ((ulong)hi << 32);
        }

        private static string ReadUtf8NullTerminated(byte[] buffer, int heapOffset, int heapSize, int index)
        {
            if (index <= 0 || index >= heapSize)
            {
                return string.Empty;
            }

            int start = heapOffset + index;
            int maxEnd = Math.Min(buffer.Length, heapOffset + heapSize);
            if (start >= maxEnd)
            {
                return string.Empty;
            }

            int end = start;
            while (end < maxEnd && buffer[end] != 0)
            {
                end++;
            }

            return Encoding.UTF8.GetString(buffer, start, end - start);
        }

        private sealed class RawTypeDef
        {
            public RawTypeDef(string ns, string name, int fieldList, int methodList)
            {
                Namespace = ns;
                Name = name;
                FieldList = fieldList;
                MethodList = methodList;
            }

            public string Namespace { get; }
            public string Name { get; }
            public int FieldList { get; }
            public int MethodList { get; }
        }

        private sealed class DiscoveredMethodRecord
        {
            public DiscoveredMethodRecord(
                string displayName,
                string simpleName,
                IReadOnlyList<string> hoistedLocals,
                bool isStateMachine)
            {
                DisplayName = displayName;
                SimpleName = simpleName;
                HoistedLocals = hoistedLocals;
                IsStateMachine = isStateMachine;
            }

            public string DisplayName { get; }
            public string SimpleName { get; }
            public IReadOnlyList<string> HoistedLocals { get; }
            public bool IsStateMachine { get; }
        }
    }
}
