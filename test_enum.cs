using System;

namespace TestNamespace
{
    /// <summary>
    /// Test enum for verification
    /// </summary>
    public enum Color
    {
        /// <summary>
        /// Red color
        /// </summary>
        Red,
        /// <summary>
        /// Green color
        /// </summary>
        Green = 1,
        /// <summary>
        /// Blue color
        /// </summary>
        Blue = 2
    }

    public class TestClass
    {
        public Color BackgroundColor { get; set; }

        public void SetColor(Color color)
        {
            BackgroundColor = color;
        }
    }
}