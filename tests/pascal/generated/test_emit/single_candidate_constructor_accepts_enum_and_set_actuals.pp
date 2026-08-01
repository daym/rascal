unit u;
interface
type
  TRegisterType = (rt_int);
  TRegSet = set of byte;
  TUsedRegs = class
    constructor Create_Regset(aTyp : TRegisterType; const Regs : TRegSet);
    function GetUsedRegs : TRegSet;
  end;
  TAllUsedRegs = array[TRegisterType] of TUsedRegs;
var UsedRegs : TAllUsedRegs;
procedure CopyUsedRegs(var dest : TAllUsedRegs);
implementation
constructor TUsedRegs.Create_Regset(aTyp : TRegisterType; const Regs : TRegSet);
begin
end;
function TUsedRegs.GetUsedRegs : TRegSet;
begin
  Result := [];
end;
procedure CopyUsedRegs(var dest : TAllUsedRegs);
var i : TRegisterType;
begin
  dest[i] := TUsedRegs.Create_Regset(i, UsedRegs[i].GetUsedRegs);
end;
end.
