using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEngine;

namespace DscSim
{
    // 書き出しとトレース記録で共通の、プロジェクト内の場所とゲームの準備
    internal static class SimProject
    {
        public const string SelectScenePath = "Assets/Scenes/SelectScene.unity";

        public sealed class Character
        {
            public string Name;
            public string PrefabPath;
            public GameObject Prefab;
        }

        public static string ProjectRoot() => Directory.GetParent(Application.dataPath).FullName;

        // 書き出し・シナリオ・トレースの置き場所。環境変数 DSC_DATA があればそちら
        public static string DataDir()
        {
            string env = Environment.GetEnvironmentVariable("DSC_DATA");
            return string.IsNullOrEmpty(env) ? Path.Combine(ProjectRoot(), "rust", "data") : env;
        }

        public static string StageScenePath(string stage) => $"Assets/Scenes/Stage/{stage}Scene.unity";

        // キャラ選択画面に並んでいるキャラ（MovePlayer を持つPrefab）。名前はPrefabのファイル名の小文字
        public static List<Character> SelectableCharacters()
        {
            return AssetDatabase.GetDependencies(SelectScenePath, false)
                .Where(p => p.EndsWith(".prefab", StringComparison.OrdinalIgnoreCase))
                .Select(p => new Character
                {
                    Name = Path.GetFileNameWithoutExtension(p).ToLowerInvariant(),
                    PrefabPath = p,
                    Prefab = AssetDatabase.LoadAssetAtPath<GameObject>(p),
                })
                .Where(c => c.Prefab != null && c.Prefab.GetComponent<MovePlayer>() != null)
                .OrderBy(c => c.Name, StringComparer.Ordinal)
                .ToList();
        }

        // 通常はタイトル画面などで作られる常駐オブジェクトを用意する
        public static void EnsureManagers()
        {
            if (InputManager.Instance == null)
                new GameObject("InputManager").AddComponent<InputManager>();
            InputManager.Instance.inputP1 = new NullInputProvider();
            InputManager.Instance.inputP2 = new NullInputProvider();

            if (PlayerSelection.Instance == null)
                new GameObject("PlayerSelection").AddComponent<PlayerSelection>();
        }
    }
}
