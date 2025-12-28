using UnityEngine;
using System;
using System.Collections.Generic;

public class UnityMainThreadDispatcher : MonoBehaviour
{
    static readonly Queue<Action> queue = new Queue<Action>();

    public static void Enqueue(Action action)
    {
        lock (queue)
        {
            queue.Enqueue(action);
        }
    }

    void Update()
    {
        lock (queue)
        {
            while (queue.Count > 0)
                queue.Dequeue()?.Invoke();
        }
    }
}
