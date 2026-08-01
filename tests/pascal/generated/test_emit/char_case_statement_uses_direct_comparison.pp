unit u;
interface
type
  tchar = char;
  tcallback = procedure(p: tchar; arg: pointer) of object;
  tlist = class
    procedure foreachcall(cb : tcallback; arg : pointer);
  end;
  thost = class
    ch : tchar;
    list : tlist;
    procedure run;
  end;
implementation
procedure tlist.foreachcall(cb : tcallback; arg : pointer); begin end;
procedure thost.run;
var c : tchar;
begin
  c := 'a';
  case ch of
    'a' : c := 'b';
    'b', 'c' : c := 'd';
  else
    c := 'z';
  end;
end;
end.
