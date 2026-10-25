using System;
using System.Diagnostics;
using System.Text;
using System.Threading;

class TrialControl {
    static int Main(string[] args) {
        Console.OutputEncoding = Encoding.UTF8;
        string mode = args.Length == 0 ? "success" : args[0];
        if (mode == "child") { Thread.Sleep(60000); return 0; }
        if (mode == "orphan" || mode == "limit") {
            int count = mode == "limit" ? 2 : 1;
            for (int i = 0; i < count; i++) {
                var info = new ProcessStartInfo(Environment.GetEnvironmentVariable("ELY_LOK_WORKER"), "child");
                info.UseShellExecute = false;
                info.CreateNoWindow = true;
                using (var child = Process.Start(info)) Console.WriteLine("owned-worker=" + child.Id);
            }
        }
        if (mode == "limit") Thread.Sleep(60000);
        Console.WriteLine("synthetic-control=" + mode);
        Console.Error.WriteLine("synthetic-stderr=" + mode);
        return 0;
    }
}
