using System;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading.Tasks;
using Microsoft.Win32.SafeHandles;

public sealed class ElyTrialJob : IDisposable {
    [StructLayout(LayoutKind.Sequential)] struct Security {
        public int size;
        public IntPtr descriptor;
        [MarshalAs(UnmanagedType.Bool)] public bool inherit;
    }
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct Startup {
        public int size;
        public string reserved, desktop, title;
        public int x, y, width, height, xChars, yChars, fill, flags;
        public short show, reservedSize;
        public IntPtr reservedData, input, output, error;
    }
    [StructLayout(LayoutKind.Sequential)] struct ExtendedStartup {
        public Startup startup;
        public IntPtr attributes;
    }
    [StructLayout(LayoutKind.Sequential)] struct ProcessInfo {
        public IntPtr process, thread;
        public uint pid, tid;
    }
    [StructLayout(LayoutKind.Sequential)] struct BasicLimits {
        public long processTime, jobTime;
        public uint flags;
        public UIntPtr minimumWorkingSet, maximumWorkingSet;
        public uint processes;
        public UIntPtr affinity;
        public uint priority, scheduling;
    }
    [StructLayout(LayoutKind.Sequential)] struct Limits {
        public BasicLimits basic;
        public ulong readOperations, writeOperations, otherOperations, readBytes, writeBytes, otherBytes;
        public UIntPtr processMemory, jobMemory, peakProcessMemory, peakJobMemory;
    }
    [StructLayout(LayoutKind.Sequential)] struct Accounting {
        public long userTime, kernelTime, periodUserTime, periodKernelTime;
        public uint faults, total, active, terminated;
    }
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateJobObjectW(IntPtr security, string name);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job, int kind, ref Limits value, uint size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool QueryInformationJobObject(IntPtr job, int kind, out Accounting value, uint size, IntPtr returned);
    [DllImport("kernel32.dll", EntryPoint="QueryInformationJobObject", SetLastError=true)] static extern bool QueryIds(IntPtr job, int kind, IntPtr value, uint size, out uint returned);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateJobObject(IntPtr job, uint code);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool CreatePipe(out IntPtr read, out IntPtr write, ref Security security, uint size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool SetHandleInformation(IntPtr handle, uint mask, uint flags);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool InitializeProcThreadAttributeList(IntPtr attributes, int count, uint flags, ref IntPtr size);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool UpdateProcThreadAttribute(IntPtr attributes, uint flags, IntPtr attribute, IntPtr value, IntPtr size, IntPtr previous, IntPtr returned);
    [DllImport("kernel32.dll")] static extern void DeleteProcThreadAttributeList(IntPtr attributes);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern IntPtr CreateFileW(string name, uint access, uint sharing, ref Security security, uint creation, uint flags, IntPtr template);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] static extern bool CreateProcessW(string application, StringBuilder command, IntPtr processSecurity, IntPtr threadSecurity, bool inherit, uint flags, IntPtr environment, string directory, ref ExtendedStartup startup, out ProcessInfo process);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
    [DllImport("kernel32.dll", SetLastError=true)] static extern bool TerminateProcess(IntPtr process, uint code);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);

    IntPtr job;
    StreamReader output, error;
    public Process Root { get; private set; }
    public Task<string> Stdout { get; private set; }
    public Task<string> Stderr { get; private set; }

    static void Check(bool success) {
        if (!success) throw new Win32Exception(Marshal.GetLastWin32Error());
    }
    static void Close(ref IntPtr handle) {
        if (handle != IntPtr.Zero && handle != (IntPtr)(-1)) CloseHandle(handle);
        handle = IntPtr.Zero;
    }
    static string Quote(string argument) {
        var value = new StringBuilder("\"");
        int slashes = 0;
        foreach (char c in argument) {
            if (c == '\\') { slashes++; continue; }
            value.Append('\\', c == '"' ? slashes * 2 + 1 : slashes);
            value.Append(c);
            slashes = 0;
        }
        return value.Append('\\', slashes * 2).Append('"').ToString();
    }
    static StreamReader Reader(ref IntPtr handle) {
        var owned = new SafeFileHandle(handle, true);
        handle = IntPtr.Zero;
        try { return new StreamReader(new FileStream(owned, FileAccess.Read, 4096, false), Encoding.UTF8); }
        catch { owned.Dispose(); throw; }
    }
    public static ElyTrialJob Start(ProcessStartInfo info) {
        var result = new ElyTrialJob();
        var security = new Security { size=Marshal.SizeOf<Security>(), inherit=true };
        IntPtr outRead=IntPtr.Zero, outWrite=IntPtr.Zero, errRead=IntPtr.Zero, errWrite=IntPtr.Zero;
        IntPtr input=IntPtr.Zero, attributes=IntPtr.Zero, handles=IntPtr.Zero, environment=IntPtr.Zero;
        var process = new ProcessInfo();
        bool initialized = false, started = false;
        try {
            result.job = CreateJobObjectW(IntPtr.Zero, null);
            Check(result.job != IntPtr.Zero);
            var limits = new Limits { basic=new BasicLimits { flags=0x2000 } };
            Check(SetInformationJobObject(result.job, 9, ref limits, (uint)Marshal.SizeOf<Limits>()));
            Check(CreatePipe(out outRead, out outWrite, ref security, 0));
            Check(CreatePipe(out errRead, out errWrite, ref security, 0));
            Check(SetHandleInformation(outRead, 1, 0));
            Check(SetHandleInformation(errRead, 1, 0));
            input = CreateFileW("NUL", 0x80000000, 3, ref security, 3, 0x80, IntPtr.Zero);
            Check(input != (IntPtr)(-1));
            IntPtr bytes = IntPtr.Zero;
            InitializeProcThreadAttributeList(IntPtr.Zero, 1, 0, ref bytes);
            attributes = Marshal.AllocHGlobal(bytes);
            Check(InitializeProcThreadAttributeList(attributes, 1, 0, ref bytes));
            initialized = true;
            handles = Marshal.AllocHGlobal(IntPtr.Size * 3);
            Marshal.WriteIntPtr(handles, 0, input);
            Marshal.WriteIntPtr(handles, IntPtr.Size, outWrite);
            Marshal.WriteIntPtr(handles, IntPtr.Size * 2, errWrite);
            Check(UpdateProcThreadAttribute(attributes, 0, (IntPtr)0x20002, handles, (IntPtr)(IntPtr.Size * 3), IntPtr.Zero, IntPtr.Zero));
            var startup = new ExtendedStartup {
                startup=new Startup { size=Marshal.SizeOf<ExtendedStartup>(), flags=0x100, input=input, output=outWrite, error=errWrite },
                attributes=attributes
            };
            string block = String.Join("\0", info.Environment.OrderBy(pair => pair.Key, StringComparer.OrdinalIgnoreCase).Select(pair => pair.Key + "=" + pair.Value)) + "\0\0";
            environment = Marshal.StringToHGlobalUni(block);
            var command = new StringBuilder(Quote(info.FileName));
            foreach (string argument in info.ArgumentList) command.Append(' ').Append(Quote(argument));
            Check(CreateProcessW(info.FileName, command, IntPtr.Zero, IntPtr.Zero, true, 0x08080404, environment,
                String.IsNullOrEmpty(info.WorkingDirectory) ? null : info.WorkingDirectory, ref startup, out process));
            started = true;
            Check(AssignProcessToJobObject(result.job, process.process));
            result.Root = Process.GetProcessById((int)process.pid);
            var rootHandle = result.Root.Handle;
            result.output = Reader(ref outRead);
            result.error = Reader(ref errRead);
            result.Stdout = result.output.ReadToEndAsync();
            result.Stderr = result.error.ReadToEndAsync();
            Close(ref outWrite);
            Close(ref errWrite);
            Check(ResumeThread(process.thread) != UInt32.MaxValue);
            return result;
        } catch {
            if (started) TerminateProcess(process.process, 1);
            result.Dispose();
            throw;
        } finally {
            Close(ref process.thread);
            Close(ref process.process);
            Close(ref outRead); Close(ref outWrite); Close(ref errRead); Close(ref errWrite); Close(ref input);
            if (initialized) DeleteProcThreadAttributeList(attributes);
            if (attributes != IntPtr.Zero) Marshal.FreeHGlobal(attributes);
            if (handles != IntPtr.Zero) Marshal.FreeHGlobal(handles);
            if (environment != IntPtr.Zero) Marshal.FreeHGlobal(environment);
        }
    }
    public uint Active {
        get {
            Accounting value;
            Check(QueryInformationJobObject(job, 1, out value, (uint)Marshal.SizeOf<Accounting>(), IntPtr.Zero));
            return value.active;
        }
    }
    public void Stop() { Check(TerminateJobObject(job, 1)); }
    public uint[] ProcessIds {
        get {
            IntPtr buffer = Marshal.AllocHGlobal(8192);
            try {
                uint returned;
                Check(QueryIds(job, 3, buffer, 8192, out returned));
                int count = Marshal.ReadInt32(buffer, 4);
                var ids = new uint[count];
                for (int i = 0; i < count; i++) ids[i] = (uint)Marshal.ReadIntPtr(buffer, 8 + i * IntPtr.Size).ToInt64();
                return ids;
            } finally { Marshal.FreeHGlobal(buffer); }
        }
    }
    public void Dispose() {
        Close(ref job);
        if (output != null) output.Dispose();
        if (error != null) error.Dispose();
        if (Root != null) Root.Dispose();
    }
}
