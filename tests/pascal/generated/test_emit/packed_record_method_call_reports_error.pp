unit u;
interface
type
  tobj = object
    procedure ping;
  end;
  trec = packed record
    tag : byte;
    child : tobj;
  end;
procedure run;
implementation
var
  r : trec;
procedure tobj.ping;
begin
end;
procedure run;
begin
  r.child.ping;
end;
end.
