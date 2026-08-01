unit u;
interface
type
  tkind = (ka, kb);
  torder = (oa, ob);
  topt = (a, b);
  topts = set of topt;
  tobj = class
    function take(kind : tkind; name : shortstring = '';
                  order : torder = oa) : longint; overload;
    function take(name : shortstring; align : shortint;
                  opts : topts;
                  discard : boolean = true) : longint; overload;
  end;
  thost = class
    inner : tobj;
    procedure run;
  end;
implementation
function tobj.take(kind : tkind; name : shortstring;
                   order : torder) : longint;
begin take := 0; end;
function tobj.take(name : shortstring; align : shortint;
                   opts : topts; discard : boolean) : longint;
begin take := 0; end;
procedure thost.run;
var s : shortstring;
    n : longint;
    e : topts;
    r : longint;
begin
  with inner do
    r := take(s, n, e);
end;
end.
