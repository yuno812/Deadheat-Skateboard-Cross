using UnityEngine;
using System.Collections.Generic;

public class DebuffArea : MonoBehaviour
{
    public enum DebuffType
    {
        Move,
        Jump
    }

    [SerializeField] private DebuffType debuffType;
    [SerializeField] private float multiplier = 0.5f;

    [Header("自分が入ったときのバフ倍率")]
    [SerializeField] private float buffMultiplier = 1.2f;

    // このエリアを生成したプレイヤー
    private MovePlayer owner;

    // エリア内にいるプレイヤーと、そのプレイヤーのコライダー数
    private Dictionary<MovePlayer, int> playersInside =
        new Dictionary<MovePlayer, int>();

    // AreaDebuffUltから生成者を受け取る
    public void SetOwner(MovePlayer player)
    {
        owner = player;
    }

    private void OnTriggerEnter2D(Collider2D other)
    {
        MovePlayer player = other.GetComponentInParent<MovePlayer>();

        if (player == null) return;

        // 同じプレイヤーの別コライダーが入った場合
        if (playersInside.ContainsKey(player))
        {
            playersInside[player]++;
            return;
        }

        playersInside.Add(player, 1);

        // 生成者本人か相手かで倍率を変更
        float effectMultiplier =
            player == owner ? buffMultiplier : multiplier;

        ApplyEffect(player, effectMultiplier);
    }

    private void OnTriggerExit2D(Collider2D other)
    {
        MovePlayer player = other.GetComponentInParent<MovePlayer>();

        if (player == null || !playersInside.ContainsKey(player))
            return;

        playersInside[player]--;

        // まだ別のコライダーがエリア内にある場合は解除しない
        if (playersInside[player] > 0)
            return;

        RemoveEffect(player);
        playersInside.Remove(player);
    }

    private void ApplyEffect(MovePlayer player, float value)
    {
        if (debuffType == DebuffType.Move)
        {
            player.SetMoveDebuff(value);
            player.SetRotateDebuff(value);
        }
        else
        {
            player.SetJumpDebuff(value);
            Rigidbody2D rb = player.GetComponent<Rigidbody2D>();
            rb.gravityScale = rb.gravityScale / value;
        }
    }

    private void RemoveEffect(MovePlayer player)
    {
        if (debuffType == DebuffType.Move)
        {
            player.SetMoveDebuff(1f);
            player.SetRotateDebuff(1f);
        }
        else
        {
            player.SetJumpDebuff(1f);
            Rigidbody2D rb = player.GetComponent<Rigidbody2D>();
            rb.gravityScale = 1f;
        }
    }

    private void OnDestroy()
    {
        // エリアが消えたとき、残っている効果を解除
        foreach (var player in playersInside.Keys)
        {
            if (player != null)
                RemoveEffect(player);
                Rigidbody2D rb = player.GetComponent<Rigidbody2D>();
                rb.gravityScale = 1f;
        }

        playersInside.Clear();
    }
}