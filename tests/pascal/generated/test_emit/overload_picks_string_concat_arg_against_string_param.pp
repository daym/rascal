unit u;
interface
type tobj = class
  function take(kind : longint;
                name : shortstring = '') : longint; overload;
  function take(name : shortstring;
                count : longint = 0) : longint; overload;
end;
procedure run(o : tobj; suffix : shortstring);
implementation
function tobj.take(kind : longint; name : shortstring) : longint;
begin take := 0; end;
function tobj.take(name : shortstring; count : longint) : longint;
begin take := 0; end;
procedure run(o : tobj; suffix : shortstring);
var r : longint;
begin
  r := o.take('*' + suffix);
end;
end.
