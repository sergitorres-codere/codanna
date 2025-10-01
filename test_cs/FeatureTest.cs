using System;
using System.Collections.Generic;
using System.Linq;

namespace TestApp
{
    // Interface
    public interface IService
    {
        void Execute();
        int GetValue();
    }

    // Class implementing interface
    public class ServiceImpl : IService
    {
        private int value = 42;

        public void Execute()
        {
            Console.WriteLine("Executing");
            var helper = new Helper();
            helper.DoWork();
        }

        public int GetValue() => value;
    }

    // Struct
    public struct Point
    {
        public int X { get; set; }
        public int Y { get; set; }

        public Point(int x, int y)
        {
            X = x;
            Y = y;
        }
    }

    // Enum
    public enum Status
    {
        Active,
        Inactive,
        Pending
    }

    // Record (C# 9+)
    public record Person(string Name, int Age);

    // Static class with extension method
    public static class Extensions
    {
        public static bool IsValid(this string str) => !string.IsNullOrEmpty(str);
    }

    // Generic class
    public class Container<T>
    {
        private T item;

        public void Store(T value) => item = value;
        public T Retrieve() => item;
    }

    // Internal class
    internal class Helper
    {
        public void DoWork()
        {
            Console.WriteLine("Working");
        }
    }

    // Abstract class
    public abstract class BaseService
    {
        protected abstract void Initialize();

        public virtual void Start()
        {
            Initialize();
        }
    }
}