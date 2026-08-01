unit u;
interface
type
  fvmlib = record
    x : longint;
  end;
  tcmd = record
    fvmlib : fvmlib;
  end;
procedure run(var c : tcmd);
implementation
procedure run(var c : tcmd);
begin
  c.fvmlib.x := 7;
end;
end.
