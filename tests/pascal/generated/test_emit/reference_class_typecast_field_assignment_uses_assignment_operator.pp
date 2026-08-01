unit u;
interface
type
  tbox = record
    n : longint;
  end;
  tbase = class end;
  tchild = class(tbase)
    value : tbox;
  end;
operator :=(const n : longint) : tbox;
procedure store(p : tbase; n : longint);
implementation
operator :=(const n : longint) : tbox;
begin
  result.n := n;
end;
procedure store(p : tbase; n : longint);
begin
  tchild(p).value := n;
end;
end.
