using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class ElyNativeUI {
 public delegate bool EnumProc(IntPtr h,IntPtr data);
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc p,IntPtr data);
 [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr h,EnumProc p,IntPtr data);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassNameW(IntPtr h,StringBuilder b,int n);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowTextW(IntPtr h,StringBuilder b,int n);
 [DllImport("user32.dll")] static extern int GetDlgCtrlID(IntPtr h);
 [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll",EntryPoint="SendMessageTimeoutW",SetLastError=true)] static extern IntPtr SendTimeout(IntPtr h,uint m,IntPtr w,IntPtr l,uint f,uint t,out IntPtr result);
 public static IntPtr SendMessageW(IntPtr h,uint m,IntPtr w,IntPtr l) {
  IntPtr result;if(SendTimeout(h,m,w,l,2,3000,out result)==IntPtr.Zero)throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(),"Owned window message timed out");return result;
 }
 [DllImport("user32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr SendMessageTimeoutW(IntPtr h,uint m,IntPtr w,string l,uint f,uint t,out IntPtr result);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h,ref Position p);
 [DllImport("user32.dll",SetLastError=true)] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
 [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h,IntPtr dc,uint f);
 [DllImport("user32.dll")] static extern uint MapVirtualKeyW(uint code,uint kind);
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int L,T,R,B;}
 [StructLayout(LayoutKind.Sequential)] struct Position {public int X,Y;}
 static IntPtr PhysicalContext() {
  var previous=SetThreadDpiAwarenessContext((IntPtr)(-4));
  if(previous==IntPtr.Zero)throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(),"Physical DPI context failed");
  return previous;
 }
 static void RestoreContext(IntPtr previous) {
  if(SetThreadDpiAwarenessContext(previous)==IntPtr.Zero)throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(),"Restoring DPI context failed");
 }
 public static Rect PhysicalBounds(IntPtr h) {
  var previous=PhysicalContext();
  try {Rect r;if(!GetWindowRect(h,out r))throw new Exception("Missing physical bounds");return r;}
  finally {RestoreContext(previous);}
 }
 public static string Geometry(IntPtr h) {
  var previous=PhysicalContext();
  try {Rect r,c;var p=new Position();GetWindowRect(h,out r);GetClientRect(h,out c);ClientToScreen(h,ref p);
   return "window="+(r.R-r.L)+"x"+(r.B-r.T)+";client="+c.R+"x"+c.B+";offset="+(p.X-r.L)+","+(p.Y-r.T);}
  finally {RestoreContext(previous);}
 }
 public static void Frame(IntPtr h) {for(int i=0;i<3;i++){SendMessageW(h,0xF,IntPtr.Zero,IntPtr.Zero);System.Threading.Thread.Sleep(80);}}
 public static string Class(IntPtr h){var b=new StringBuilder(128);GetClassNameW(h,b,128);return b.ToString();}
 public static string Title(IntPtr h){var b=new StringBuilder(512);GetWindowTextW(h,b,512);return b.ToString();}
 public static IntPtr Child(IntPtr parent,int id,string cls) {
  IntPtr found=IntPtr.Zero;
  EnumChildWindows(parent,(h,d)=>{if(GetDlgCtrlID(h)==id&&(cls==null||Class(h)==cls)){found=h;return false;}return true;},IntPtr.Zero);
  return found;
 }
 public static IntPtr Dialog(uint pid,int child,string cls) {
  IntPtr found=IntPtr.Zero;
  EnumWindows((h,d)=>{uint owner;GetWindowThreadProcessId(h,out owner);
   if(owner==pid&&Class(h)=="#32770"&&Child(h,child,cls)!=IntPtr.Zero){found=h;return false;}return true;
  },IntPtr.Zero);return found;
 }
 public static string Dump(uint pid) {
  var b=new StringBuilder();
  EnumWindows((h,d)=>{uint owner;GetWindowThreadProcessId(h,out owner);
   if(owner==pid){b.AppendLine(h+" "+Class(h)+" "+Title(h));
    if(Class(h)=="#32770")EnumChildWindows(h,(c,x)=>{b.AppendLine("  "+c+" "+GetDlgCtrlID(c)+" "+Class(c)+" "+Title(c));return true;},IntPtr.Zero);
   }return true;},IntPtr.Zero);return b.ToString();
 }
 public static IntPtr NoButton(IntPtr dialog) {
  IntPtr found=IntPtr.Zero;
  EnumChildWindows(dialog,(h,d)=>{var title=Title(h).Replace("&","");
   if(Class(h)=="Button"&&(title.StartsWith("否")||title.StartsWith("No"))){found=h;return false;}return true;
  },IntPtr.Zero);return found;
 }
 public static IntPtr Confirmation(uint pid) {
  IntPtr found=IntPtr.Zero;
  EnumWindows((h,d)=>{uint owner;GetWindowThreadProcessId(h,out owner);
   if(owner==pid&&Class(h)=="#32770"&&NoButton(h)!=IntPtr.Zero){found=h;return false;}return true;
  },IntPtr.Zero);return found;
 }
 public static void Text(IntPtr h,string text) {
  IntPtr result;
  SendMessageW(h,0xB1,IntPtr.Zero,(IntPtr)(-1));
  if(SendMessageTimeoutW(h,0xC2,(IntPtr)1,text,2,3000,out result)==IntPtr.Zero)
   throw new Exception("Setting owned filename failed: "+Marshal.GetLastWin32Error());
 }
 public static void Command(IntPtr h,int id){if(!PostMessageW(h,0x111,(IntPtr)id,IntPtr.Zero))throw new Exception("Owned command refused");}
 public static void Close(IntPtr h){if(!PostMessageW(h,0x10,IntPtr.Zero,IntPtr.Zero))throw new Exception("Owned close refused");}
 public static void Key(IntPtr h,uint code) {
  uint scan=MapVirtualKeyW(code,4);
  if(scan==0)throw new Exception("Unmapped test key");
  uint bits=1|((scan&255)<<16)|((scan>>8)!=0 ? 0x1000000u : 0u);
  if(!PostMessageW(h,0x100,(IntPtr)code,(IntPtr)(long)bits)||!PostMessageW(h,0x101,(IntPtr)code,(IntPtr)(long)(bits|0xC0000000u)))throw new Exception("Owned key post failed");
 }
 public static void Click(IntPtr h,int x,int y){
  var previous=PhysicalContext();
  try{var p=(IntPtr)((y<<16)|x);SendMessageW(h,0x200,IntPtr.Zero,p);SendMessageW(h,0x201,(IntPtr)1,p);SendMessageW(h,0x202,IntPtr.Zero,p);}
  finally{RestoreContext(previous);}
 }
}
