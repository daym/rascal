unit u;
interface
type
  tsym = class
  end;
  tlabel = class(tsym)
  end;
procedure take(var s : tsym);
procedure use(l : tlabel);
implementation
procedure take(var s : tsym);
begin
end;
procedure use(l : tlabel);
begin
  take(tsym(l));
end;
end.
