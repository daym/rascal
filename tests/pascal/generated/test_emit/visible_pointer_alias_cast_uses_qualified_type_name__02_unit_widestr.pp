unit widestr;
interface
type
  compilerwidestring = array[0..3] of widechar;
  pcompilerwidestring = ^compilerwidestring;
procedure copywidestring(src, dst : pcompilerwidestring);
implementation
procedure copywidestring(src, dst : pcompilerwidestring);
begin
end;
end.
