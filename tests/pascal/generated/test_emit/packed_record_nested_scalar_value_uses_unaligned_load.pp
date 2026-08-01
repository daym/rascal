unit u;
interface
type
  tsub = record
    x : longint;
  end;
  trec = packed record
    tag : byte;
    sub : tsub;
  end;
procedure run;
implementation
var
  r : trec;
  i : longint;
procedure run;
begin
  i := r.sub.x;
end;
end.
