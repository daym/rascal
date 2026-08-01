unit u;
interface
type
  tsym = class
  end;
  ttypesym = class(tsym)
    typedef : longint;
  end;
procedure run(tsym : ttypesym);
implementation
procedure run(tsym : ttypesym);
begin
  tsym.typedef := 3;
end;
end.
