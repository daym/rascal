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
procedure take(var x : longint);
begin
end;
procedure run;
begin
  take(r.sub.x);
end;
end.
