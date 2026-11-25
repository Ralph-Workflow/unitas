using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Debugging
{
    internal static class ConditionEvaluator
    {
        private const int MaxLength = 200;
        private const int MaxDepth = 16;

        public static bool Evaluate(string expression, IReadOnlyList<string> names, IReadOnlyList<object?> values)
        {
            if (string.IsNullOrWhiteSpace(expression) || expression.Length > MaxLength)
            {
                return false;
            }
            try
            {
                return new Parser(expression, names, values, false).ParseExpression();
            }
            catch (FormatException)
            {
                return false;
            }
        }

        public static bool ContainsSideEffectOperator(string expression)
        {
            if (string.IsNullOrEmpty(expression))
            {
                return false;
            }
            bool inSingleQuote = false;
            bool inDoubleQuote = false;
            for (int index = 0; index < expression.Length; index++)
            {
                char current = expression[index];
                if (current == '\'' && !inDoubleQuote)
                {
                    inSingleQuote = !inSingleQuote;
                    continue;
                }
                if (current == '"' && !inSingleQuote)
                {
                    inDoubleQuote = !inDoubleQuote;
                    continue;
                }
                if (inSingleQuote || inDoubleQuote)
                {
                    continue;
                }
                if (current == '+' && index + 1 < expression.Length && expression[index + 1] == '+')
                {
                    return true;
                }
                if (current == '-' && index + 1 < expression.Length && expression[index + 1] == '-')
                {
                    return true;
                }
                if (current == '=')
                {
                    char previous = index > 0 ? expression[index - 1] : '\0';
                    char next = index + 1 < expression.Length ? expression[index + 1] : '\0';
                    if (next == '=')
                    {
                        index++;
                        continue;
                    }
                    if (previous == '=' || previous == '!' || previous == '<' || previous == '>')
                    {
                        continue;
                    }
                    return true;
                }
            }
            return false;
        }

        public static bool TryEvaluateValue(
            string expression,
            IReadOnlyList<string> names,
            IReadOnlyList<object?> values,
            out object? result,
            out string error)
        {
            result = null;
            if (string.IsNullOrWhiteSpace(expression))
            {
                error = "Expression cannot be empty.";
                return false;
            }
            if (expression.Length > MaxLength)
            {
                error = "Expression exceeds maximum length.";
                return false;
            }
            if (ContainsSideEffectOperator(expression))
            {
                error = "Side-effecting evaluation is not supported in read-only debug evaluation.";
                return false;
            }
            string trimmed = expression.Trim();
            if (IsPlainIdentifier(trimmed))
            {
                if (trimmed == "true")
                {
                    result = true;
                    error = string.Empty;
                    return true;
                }
                if (trimmed == "false")
                {
                    result = false;
                    error = string.Empty;
                    return true;
                }
                if (trimmed == "null")
                {
                    result = null;
                    error = string.Empty;
                    return true;
                }
                for (int index = 0; index < names.Count; index++)
                {
                    if (string.Equals(names[index], trimmed, StringComparison.Ordinal))
                    {
                        result = index < values.Count ? values[index] : null;
                        error = string.Empty;
                        return true;
                    }
                }
                error = "Unknown local variable '" + trimmed + "'.";
                return false;
            }
            try
            {
                Parser parser = new Parser(trimmed, names, values, true);
                result = parser.ParseValueExpression();
                error = string.Empty;
                return true;
            }
            catch (FormatException exception)
            {
                error = exception.Message;
                return false;
            }
        }

        private static bool IsPlainIdentifier(string text)
        {
            if (text.Length == 0 || (!char.IsLetter(text[0]) && text[0] != '_'))
            {
                return false;
            }
            for (int index = 1; index < text.Length; index++)
            {
                if (!char.IsLetterOrDigit(text[index]) && text[index] != '_')
                {
                    return false;
                }
            }
            return true;
        }

        private sealed class Parser
        {
            private readonly string _text;
            private readonly IReadOnlyList<string> _names;
            private readonly IReadOnlyList<object?> _values;
            private readonly bool _requireKnownIdentifiers;
            private int _position;
            private int _depth;

            public Parser(string text, IReadOnlyList<string> names, IReadOnlyList<object?> values, bool requireKnownIdentifiers)
            {
                _text = text;
                _names = names;
                _values = values;
                _requireKnownIdentifiers = requireKnownIdentifiers;
            }

            public bool ParseExpression()
            {
                bool result = ParseOr();
                SkipWhitespace();
                if (_position != _text.Length)
                {
                    throw new FormatException("trailing input");
                }
                return result;
            }

            public object? ParseValueExpression()
            {
                if (!ContainsLogicalOrComparisonTokens())
                {
                    object? literal = ParseOperand();
                    SkipWhitespace();
                    if (_position == _text.Length)
                    {
                        return literal;
                    }
                    _position = 0;
                    _depth = 0;
                }
                return ParseExpression();
            }

            private bool ContainsLogicalOrComparisonTokens()
            {
                bool inSingleQuote = false;
                bool inDoubleQuote = false;
                for (int index = 0; index < _text.Length; index++)
                {
                    char current = _text[index];
                    if (current == '\'' && !inDoubleQuote)
                    {
                        inSingleQuote = !inSingleQuote;
                        continue;
                    }
                    if (current == '"' && !inSingleQuote)
                    {
                        inDoubleQuote = !inDoubleQuote;
                        continue;
                    }
                    if (inSingleQuote || inDoubleQuote)
                    {
                        continue;
                    }
                    if (current == '=' || current == '!' || current == '<' || current == '>' || current == '&' || current == '|')
                    {
                        return true;
                    }
                }
                return false;
            }

            private bool ParseOr()
            {
                bool left = ParseAnd();
                while (TryConsume("||"))
                {
                    bool right = ParseAnd();
                    left = left || right;
                }
                return left;
            }

            private bool ParseAnd()
            {
                bool left = ParseUnary();
                while (TryConsume("&&"))
                {
                    bool right = ParseUnary();
                    left = left && right;
                }
                return left;
            }

            private bool ParseUnary()
            {
                Enter();
                SkipWhitespace();
                if (_position < _text.Length && _text[_position] == '!' && !PeekEquals())
                {
                    _position++;
                    bool value = ParseUnary();
                    Leave();
                    return !value;
                }
                bool comparison = ParseComparison();
                Leave();
                return comparison;
            }

            private bool ParseComparison()
            {
                object? left = ParseOperand();
                string? op = ReadOperator();
                if (op == null)
                {
                    return Truthy(left);
                }
                object? right = ParseOperand();
                return Compare(left, op, right);
            }

            private object? ParseOperand()
            {
                SkipWhitespace();
                if (_position >= _text.Length)
                {
                    throw new FormatException("unexpected end of expression");
                }
                char current = _text[_position];
                if (current == '(')
                {
                    _position++;
                    bool value = ParseOr();
                    SkipWhitespace();
                    Expect(')');
                    return value;
                }
                if (current == '\'' || current == '"')
                {
                    return ParseString(current);
                }
                if (char.IsDigit(current) || (current == '-' && _position + 1 < _text.Length && char.IsDigit(_text[_position + 1])))
                {
                    return ParseNumber();
                }
                if (char.IsLetter(current) || current == '_')
                {
                    return ParseIdentifier();
                }
                throw new FormatException("unexpected character");
            }

            private object? ParseIdentifier()
            {
                int start = _position;
                while (_position < _text.Length && (char.IsLetterOrDigit(_text[_position]) || _text[_position] == '_'))
                {
                    _position++;
                }
                string name = _text.AsSpan(start, _position - start).ToString();
                if (name == "true")
                {
                    return true;
                }
                if (name == "false")
                {
                    return false;
                }
                if (name == "null")
                {
                    return null;
                }
                for (int index = 0; index < _names.Count; index++)
                {
                    if (string.Equals(_names[index], name, StringComparison.Ordinal))
                    {
                        return index < _values.Count ? _values[index] : null;
                    }
                }
                if (_requireKnownIdentifiers)
                {
                    throw new FormatException("Unknown local variable '" + name + "'.");
                }
                return null;
            }

            private object? ParseNumber()
            {
                int start = _position;
                if (_text[_position] == '-')
                {
                    _position++;
                }
                while (_position < _text.Length && char.IsDigit(_text[_position]))
                {
                    _position++;
                }
                if (_position < _text.Length && _text[_position] == '.')
                {
                    _position++;
                    while (_position < _text.Length && char.IsDigit(_text[_position]))
                    {
                        _position++;
                    }
                }
                return double.Parse(_text.AsSpan(start, _position - start).ToString(), CultureInfo.InvariantCulture);
            }

            private string ParseString(char quote)
            {
                _position++;
                int start = _position;
                while (_position < _text.Length && _text[_position] != quote)
                {
                    _position++;
                }
                if (_position >= _text.Length)
                {
                    throw new FormatException("unterminated string");
                }
                string value = _text.AsSpan(start, _position - start).ToString();
                _position++;
                return value;
            }

            private string? ReadOperator()
            {
                SkipWhitespace();
                if (_position >= _text.Length)
                {
                    return null;
                }
                string[] operators = new string[] { "==", "!=", "<=", ">=", "<", ">" };
                foreach (string candidate in operators)
                {
                    if (string.CompareOrdinal(_text, _position, candidate, 0, candidate.Length) == 0)
                    {
                        _position += candidate.Length;
                        return candidate;
                    }
                }
                return null;
            }

            private bool PeekEquals()
            {
                return _position + 1 < _text.Length && _text[_position + 1] == '=';
            }

            private bool TryConsume(string token)
            {
                SkipWhitespace();
                if (string.CompareOrdinal(_text, _position, token, 0, token.Length) != 0)
                {
                    return false;
                }
                _position += token.Length;
                return true;
            }

            private void Expect(char expected)
            {
                SkipWhitespace();
                if (_position >= _text.Length || _text[_position] != expected)
                {
                    throw new FormatException("expected character");
                }
                _position++;
            }

            private void SkipWhitespace()
            {
                while (_position < _text.Length && char.IsWhiteSpace(_text[_position]))
                {
                    _position++;
                }
            }

            private void Enter()
            {
                _depth++;
                if (_depth > MaxDepth)
                {
                    throw new FormatException("expression too deep");
                }
            }

            private void Leave()
            {
                _depth--;
            }

            private static bool Truthy(object? value)
            {
                return value is bool flag ? flag : value != null;
            }

            private static bool Compare(object? left, string op, object? right)
            {
                bool equal;
                int? order = null;
                if (left == null || right == null)
                {
                    equal = left == null && right == null;
                }
                else if (IsNumber(left) && IsNumber(right))
                {
                    double leftNumber = Convert.ToDouble(left, CultureInfo.InvariantCulture);
                    double rightNumber = Convert.ToDouble(right, CultureInfo.InvariantCulture);
                    equal = leftNumber == rightNumber;
                    order = leftNumber.CompareTo(rightNumber);
                }
                else if (left is string leftText && right is string rightText)
                {
                    equal = string.Equals(leftText, rightText, StringComparison.Ordinal);
                    order = string.CompareOrdinal(leftText, rightText);
                }
                else if (left is bool leftFlag && right is bool rightFlag)
                {
                    equal = leftFlag == rightFlag;
                }
                else
                {
                    equal = left.Equals(right);
                }
                switch (op)
                {
                    case "==":
                        return equal;
                    case "!=":
                        return !equal;
                    case "<":
                        return order.HasValue && order.Value < 0;
                    case "<=":
                        return order.HasValue && order.Value <= 0;
                    case ">":
                        return order.HasValue && order.Value > 0;
                    case ">=":
                        return order.HasValue && order.Value >= 0;
                    default:
                        return false;
                }
            }

            private static bool IsNumber(object? value)
            {
                return value is int || value is long || value is double || value is float
                    || value is decimal || value is short || value is byte || value is uint
                    || value is ulong || value is sbyte;
            }
        }
    }
}
