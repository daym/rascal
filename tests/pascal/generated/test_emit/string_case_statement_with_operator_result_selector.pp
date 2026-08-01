unit u;
interface
type
  tbox = record
    v : longint;
  end;
operator + (const a,b : tbox) : string;
procedure demo(a,b : tbox; var i : longint);
implementation
operator + (const a,b : tbox) : string;
begin
  result := 'ok';
end;
procedure demo(a,b : tbox; var i : longint);
begin
  case a + b of
    'ok': i := 1;
  else
    i := 0;
  end;
end;
end.
