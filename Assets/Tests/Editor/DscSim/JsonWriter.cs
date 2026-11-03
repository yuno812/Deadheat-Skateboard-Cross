using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using UnityEngine;

namespace DscSim
{
    // 書き出し用の最小限のJSONライター。JsonUtilityは入れ子の配列や辞書を書けないため自前で書く。
    // 差分を読みやすくするため、1要素1行で字下げする（Vector2だけは [x, y] を1行にまとめる）。
    internal sealed class JsonWriter
    {
        private readonly StringBuilder sb = new StringBuilder();
        private readonly Stack<bool> hasItems = new Stack<bool>();
        private bool afterKey;

        public override string ToString() => sb.ToString();

        public JsonWriter BeginObject() { BeforeValue(); sb.Append('{'); hasItems.Push(false); return this; }
        public JsonWriter EndObject() { Close('}'); return this; }
        public JsonWriter BeginArray() { BeforeValue(); sb.Append('['); hasItems.Push(false); return this; }
        public JsonWriter EndArray() { Close(']'); return this; }

        public JsonWriter Key(string name)
        {
            NewItem();
            WriteString(name);
            sb.Append(": ");
            afterKey = true;
            return this;
        }

        public JsonWriter Value(string v) { BeforeValue(); if (v == null) sb.Append("null"); else WriteString(v); return this; }
        public JsonWriter Value(float v) { BeforeValue(); sb.Append(Num(v)); return this; }
        public JsonWriter Value(int v) { BeforeValue(); sb.Append(v.ToString(CultureInfo.InvariantCulture)); return this; }
        public JsonWriter Value(bool v) { BeforeValue(); sb.Append(v ? "true" : "false"); return this; }
        public JsonWriter Value(Vector2 v) { BeforeValue(); sb.Append('[').Append(Num(v.x)).Append(", ").Append(Num(v.y)).Append(']'); return this; }

        public JsonWriter Prop(string name, string v) => Key(name).Value(v);
        public JsonWriter Prop(string name, float v) => Key(name).Value(v);
        public JsonWriter Prop(string name, int v) => Key(name).Value(v);
        public JsonWriter Prop(string name, bool v) => Key(name).Value(v);
        public JsonWriter Prop(string name, Vector2 v) => Key(name).Value(v);

        private void BeforeValue()
        {
            if (afterKey) { afterKey = false; return; }
            if (hasItems.Count > 0) NewItem();
        }

        private void NewItem()
        {
            if (hasItems.Pop()) sb.Append(',');
            hasItems.Push(true);
            sb.Append('\n').Append(' ', hasItems.Count * 2);
        }

        private void Close(char c)
        {
            if (hasItems.Pop()) sb.Append('\n').Append(' ', hasItems.Count * 2);
            sb.Append(c);
        }

        private static string Num(float v)
        {
            if (float.IsNaN(v) || float.IsInfinity(v))
                throw new ArgumentException($"JSONに書けない値です: {v}");
            return v.ToString("R", CultureInfo.InvariantCulture);
        }

        private void WriteString(string s)
        {
            sb.Append('"');
            foreach (char c in s)
            {
                switch (c)
                {
                    case '"': sb.Append("\\\""); break;
                    case '\\': sb.Append("\\\\"); break;
                    case '\n': sb.Append("\\n"); break;
                    case '\r': sb.Append("\\r"); break;
                    case '\t': sb.Append("\\t"); break;
                    default:
                        if (c < 0x20) sb.Append("\\u").Append(((int)c).ToString("x4"));
                        else sb.Append(c);
                        break;
                }
            }
            sb.Append('"');
        }
    }
}
