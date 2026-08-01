unit u;
interface
type
  topt = (a, b);
  topts = set of topt;
  tobj = class
    function take(kind : longint) : longint; overload;
    function take(name : shortstring; opts : topts;
                  discard : boolean = true) : longint; overload;
    function options : topts;
  end;
procedure run(o : tobj);
implementation
function tobj.take(kind : longint) : longint;
begin take := 0; end;
function tobj.take(name : shortstring; opts : topts;
                   discard : boolean) : longint;
begin take := 0; end;
function tobj.options : topts;
begin options := []; end;
procedure run(o : tobj);
var r : longint;
begin
  r := o.take('hello', o.options - [a]);
end;
end.
