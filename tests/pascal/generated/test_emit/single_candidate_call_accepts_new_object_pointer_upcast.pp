unit u;
interface
type
  tbase = object
  end;
  tchild = object(tbase)
    constructor init(n : longint);
  end;
  pbase = ^tbase;
  pchild = ^tchild;
procedure take(p : pbase);
procedure run;
implementation
constructor tchild.init(n : longint); begin end;
procedure take(p : pbase); begin end;
procedure run;
begin
  take(new(pchild, init(1)));
end;
end.
