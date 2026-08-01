unit u;
interface
type
  topt = (a, b);
  topts = set of topt;
  tobj = class
    function take(kind : longint) : longint; overload;
    function take(name : shortstring; align : shortint;
                  opts : topts;
                  discard : boolean = true) : longint; overload;
  end;
procedure run(o : tobj);
implementation
function tobj.take(kind : longint) : longint;
begin take := 0; end;
function tobj.take(name : shortstring; align : shortint;
                   opts : topts; discard : boolean) : longint;
begin take := 0; end;
procedure run(o : tobj);
var r : longint;
begin
  r := o.take('hello', 0, []);
end;
end.
