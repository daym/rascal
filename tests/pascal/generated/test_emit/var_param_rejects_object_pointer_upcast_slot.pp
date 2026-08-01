unit u;
interface
type
  tbase = object
  end;
  tchild = object(tbase)
  end;
  pbase = ^tbase;
  pchild = ^tchild;
procedure take(var p : pbase);
procedure run(c : pchild);
implementation
procedure take(var p : pbase); begin end;
procedure run(c : pchild);
begin
  take(c);
end;
end.
