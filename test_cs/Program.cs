using System;

namespace TestApp
{
    public class Program
    {
        public static void Main(string[] args)
        {
            var helper = new Helper();
            helper.DoWork();
            Console.WriteLine("Hello");
        }
    }

    public class Helper
    {
        public void DoWork()
        {
            Console.WriteLine("Working");
        }
    }
}