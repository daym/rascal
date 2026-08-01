unit u;
interface
type
  tinner = class
    value : byte;
  end;
  thost = class
    value : longint;
    inner : tinner;
    procedure run;
  end;
procedure take(b : byte); overload;
procedure take(i : longint); overload;
implementation
procedure take(b : byte); begin end;
procedure take(i : longint); begin end;
procedure thost.run;
begin
  with inner do
    take(value);
end;
end.
