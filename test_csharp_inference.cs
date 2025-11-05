using System;
using System.Collections.Generic;

namespace TypeInferenceTest
{
    class Program
    {
        static Helper GetHelper()
        {
            return new Helper();
        }

        static void Main(string[] args)
        {
            // Test 1: Object creation (already supported)
            var obj1 = new Helper();
            obj1.DoSomething();

            // Test 2: Method return type inference (NEW)
            var obj2 = GetHelper();
            obj2.DoSomething();

            // Test 3: Method with prefix patterns (NEW)
            var user = CreateUser();
            user.Save();

            var connection = BuildConnection();
            connection.Open();

            // Test 4: Collection indexer (NEW)
            var users = new List<User>();
            var firstUser = users[0];
            firstUser.Save();

            // Test 5: Ternary expression (NEW)
            var item = condition ? new Helper() : new Helper();
            item.DoSomething();
        }

        static User CreateUser()
        {
            return new User();
        }

        static Connection BuildConnection()
        {
            return new Connection();
        }
    }

    class Helper
    {
        public void DoSomething() { }
    }

    class User
    {
        public void Save() { }
    }

    class Connection
    {
        public void Open() { }
    }
}
