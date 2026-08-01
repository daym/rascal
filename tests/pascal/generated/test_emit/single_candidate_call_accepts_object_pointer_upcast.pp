unit u;
interface
type
  tbase = object
  end;
  tchild = object(tbase)
  end;
  pbase = ^tbase;
  pchild = ^tchild;
procedure take(p : pbase);
procedure run(c : pchild);
implementation
procedure take(p : pbase); begin end;
procedure run(c : pchild);
begin
  take(c);
end;
end.
