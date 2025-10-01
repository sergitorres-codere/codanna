using System;

namespace Test
{
    public class Program
    {
        public static void Main()
        {
            var helper = new Helper();
            helper.DoWork();
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