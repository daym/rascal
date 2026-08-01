unit u;
interface
procedure run;
implementation
procedure run;
var
  rec : packed record
    a, b, c : byte;
    payload : array[0..3] of byte;
  end;
begin
  rec.a := 1;
end;
end.
