unit rgobj;
interface
uses cgbase, cgutils;
type
  trgobj = class
    constructor create(rt : tregistertype; sub : tsubregister;
      const usable : array of tsuperregister; first : tsuperregister;
      preserved : tcpuregisterset);
    procedure alloccpuregisters(rt : tregistertype;
      const regs : tcpuregisterset);
  end;
implementation
constructor trgobj.create(rt : tregistertype; sub : tsubregister;
  const usable : array of tsuperregister; first : tsuperregister;
  preserved : tcpuregisterset);
begin
end;
procedure trgobj.alloccpuregisters(rt : tregistertype;
  const regs : tcpuregisterset);
begin
end;
end.
