unit u;
interface
type
  pwide = ^twide;
  twide = record
    len : longint;
  end;
  tvalue = record
    valueptr : pointer;
  end;
procedure init(out r : pwide);
procedure run(var value : tvalue);
implementation
procedure init(out r : pwide);
begin
  r := nil;
end;
procedure run(var value : tvalue);
begin
  init(value.valueptr);
end;
end.
