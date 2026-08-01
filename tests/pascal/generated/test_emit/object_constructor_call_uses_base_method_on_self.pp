unit u;
interface
type
  tbase = object
    constructor init(n : integer);
  end;
  tchild = object(tbase)
    constructor init(n : integer);
  end;
implementation
constructor tbase.init(n : integer);
begin
end;
constructor tchild.init(n : integer);
begin
  tbase.init(n);
end;
end.
