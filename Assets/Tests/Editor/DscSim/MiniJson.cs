using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;

namespace DscSim
{
    // シナリオ読み込み用の最小限のJSONパーサー。JsonUtilityでは「項目がない」と「0」を区別できないため自前で読む。
    // object → Dictionary<string, object>、array → List<object>、number → double、ほか string / bool / null。
    internal sealed class MiniJson
    {
        private readonly string s;
        private int i;

        private MiniJson(string s) { this.s = s; }

        public static object Parse(string text)
        {
            var p = new MiniJson(text);
            object value = p.ReadValue();
            p.SkipWhitespace();
            if (p.i != p.s.Length) throw p.Error("余分な文字があります");
            return value;
        }

        private object ReadValue()
        {
            SkipWhitespace();
            if (i >= s.Length) throw Error("値がありません");
            switch (s[i])
            {
                case '{': return ReadObject();
                case '[': return ReadArray();
                case '"': return ReadString();
                case 't': Expect("true"); return true;
                case 'f': Expect("false"); return false;
                case 'n': Expect("null"); return null;
                default: return ReadNumber();
            }
        }

        private Dictionary<string, object> ReadObject()
        {
            var result = new Dictionary<string, object>();
            i++;
            SkipWhitespace();
            if (Peek() == '}') { i++; return result; }
            while (true)
            {
                SkipWhitespace();
                string key = ReadString();
                SkipWhitespace();
                if (Peek() != ':') throw Error("':' がありません");
                i++;
                result[key] = ReadValue();
                SkipWhitespace();
                char c = Peek();
                i++;
                if (c == '}') return result;
                if (c != ',') throw Error("',' か '}' がありません");
            }
        }

        private List<object> ReadArray()
        {
            var result = new List<object>();
            i++;
            SkipWhitespace();
            if (Peek() == ']') { i++; return result; }
            while (true)
            {
                result.Add(ReadValue());
                SkipWhitespace();
                char c = Peek();
                i++;
                if (c == ']') return result;
                if (c != ',') throw Error("',' か ']' がありません");
            }
        }

        private string ReadString()
        {
            if (Peek() != '"') throw Error("文字列がありません");
            i++;
            var sb = new StringBuilder();
            while (true)
            {
                if (i >= s.Length) throw Error("文字列が閉じていません");
                char c = s[i++];
                if (c == '"') return sb.ToString();
                if (c != '\\') { sb.Append(c); continue; }
                char e = s[i++];
                switch (e)
                {
                    case '"': case '\\': case '/': sb.Append(e); break;
                    case 'b': sb.Append('\b'); break;
                    case 'f': sb.Append('\f'); break;
                    case 'n': sb.Append('\n'); break;
                    case 'r': sb.Append('\r'); break;
                    case 't': sb.Append('\t'); break;
                    case 'u': sb.Append((char)Convert.ToInt32(s.Substring(i, 4), 16)); i += 4; break;
                    default: throw Error($"不明なエスケープ \\{e}");
                }
            }
        }

        private double ReadNumber()
        {
            int start = i;
            while (i < s.Length && "+-0123456789.eE".IndexOf(s[i]) >= 0) i++;
            if (start == i) throw Error("不明な値です");
            return double.Parse(s.Substring(start, i - start), NumberStyles.Float, CultureInfo.InvariantCulture);
        }

        private void Expect(string word)
        {
            if (string.CompareOrdinal(s, i, word, 0, word.Length) != 0) throw Error($"{word} がありません");
            i += word.Length;
        }

        private char Peek() => i < s.Length ? s[i] : '\0';

        private void SkipWhitespace()
        {
            while (i < s.Length && char.IsWhiteSpace(s[i])) i++;
        }

        private FormatException Error(string msg) => new FormatException($"JSON {i}文字目: {msg}");
    }
}
