unit u;
interface
type tobj = class
  function take(name : shortstring; align : shortint;
                discard : boolean = true) : longint; overload;
  function take(kind : longint; name : shortstring = '') : longint; overload;
end;
procedure run(o : tobj);
implementation
function tobj.take(name : shortstring; align : shortint;
                   discard : boolean) : longint;
begin take := 0; end;
function tobj.take(kind : longint; name : shortstring) : longint;
begin take := 0; end;
procedure run(o : tobj);
var n : longint;
    s : shortstring;
    r : longint;
begin
  r := o.take(s, n);
end;
end.
