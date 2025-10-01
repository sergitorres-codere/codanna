using System;

namespace TestNamespace
{
    public interface ITestService
    {
        void TestMethod();
    }

    public class TestClass : ITestService
    {
        public void TestMethod()
        {
            Console.WriteLine("Hello");
            SomeOtherMethod();
        }

        private void SomeOtherMethod()
        {
            // Some implementation
        }
    }
}