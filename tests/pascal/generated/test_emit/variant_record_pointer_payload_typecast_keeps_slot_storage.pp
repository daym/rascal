unit u;
interface
type
  pwide = ^longint;
  tconstvalue = record
    case integer of
      0 : (valueptr : pointer; len : longint);
      1 : (valueord : longint);
  end;
procedure touch(var p : pwide);
procedure run(var value : tconstvalue; pw : pwide);
implementation
procedure touch(var p : pwide);
begin
end;
procedure run(var value : tconstvalue; pw : pwide);
begin
  pwide(value.valueptr) := pw;
  touch(pwide(value.valueptr));
end;
end.
