unit u;
interface
type
  trec = packed record
    tag : byte;
    name : array[0..15] of char;
  end;
procedure run;
implementation
var
  r : trec;
  c : char;
procedure run;
begin
  c := r.name[0];
end;
end.
