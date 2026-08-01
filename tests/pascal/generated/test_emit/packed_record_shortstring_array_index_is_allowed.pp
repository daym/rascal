unit u;
interface
type
  trec = packed record
    tag : byte;
    names : array[0..1] of string[20];
  end;
procedure run;
implementation
var
  r : trec;
  s : string[20];
procedure run;
begin
  s := r.names[0];
end;
end.
