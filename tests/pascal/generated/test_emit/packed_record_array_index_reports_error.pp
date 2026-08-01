unit u;
interface
type
  trec = packed record
    tag : byte;
    data : array[0..2] of longint;
  end;
procedure run;
implementation
var
  r : trec;
  i : longint;
procedure run;
begin
  i := r.data[0];
end;
end.
