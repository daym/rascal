unit u;
interface
type
  tintarray = array[0..2] of longint;
procedure take(const xs : array of longint);
procedure run;
implementation
procedure take(const xs : array of longint); begin end;
procedure run;
var
  xs : tintarray;
begin
  take(xs);
end;
end.
