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
procedure run;
begin
  r.sub.x := 1;
end;
end.
