using System;
using System.Collections.Generic;
using System.Globalization;

namespace Unitas.Debugging
{
    internal static class ConditionEvaluator
    {
        private const int MaxLength = 200;
        private const int MaxDepth = 16;

        // limit: identifiers and literals only, 200 characters, 16 levels deep, no member access and no calls
        public static bool Evaluate(string expression, IReadOnlyList<string> names, IReadOnlyList<object?> values)
        {
            if (string.IsNullOrWhiteSpace(expression) || expression.Length > MaxLength)
            {
                return false;
            }
            try
            {
                return new Parser(expression, names, values).ParseExpression();
            }
            catch (FormatException)
            {
                return false;
            }
        }

        private sealed class Parser
        {
            private readonly string _text;
            private readonly IReadOnlyList<string> _names;
            private readonly IReadOnlyList<object?> _values;
            private int _position;
            private int _depth;

            public Parser(string text, IReadOnlyList<string> names, IReadOnlyList<object?> values)
            {
                _text = text;
                _names = names;
                _values = values;
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
