unit u;
interface
type
  tbase = class
    constructor create; virtual;
  end;
  tchild = class(tbase)
    constructor create;
  end;
implementation
constructor tbase.create; begin end;
constructor tchild.create; begin end;
end.
