unit u;
interface
type
  tsym = class
  end;
  tlabel = class(tsym)
  end;
procedure demo(l : tlabel);
implementation
procedure demo(l : tlabel);
  procedure touch(var s : tsym);
  begin
  end;
begin
  touch(tsym(l));
end;
end.
