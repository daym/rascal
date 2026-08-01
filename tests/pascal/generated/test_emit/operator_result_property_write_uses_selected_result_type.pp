unit u;
interface
type
  tnum = record
    v : longint;
  end;
  tbox = class
    procedure setvalue(x : longint);
    property value : longint write setvalue;
  end;
operator + (const a,b : tnum) : tbox;
procedure demo(a,b : tnum);
implementation
operator + (const a,b : tnum) : tbox;
begin
  result := nil;
end;
procedure tbox.setvalue(x : longint);
begin
end;
procedure demo(a,b : tnum);
begin
  (a + b).value := 3;
end;
end.
