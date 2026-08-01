unit u;
interface
type
  tbase = class
    procedure seal; virtual;
  end;
  tchild = class(tbase)
    procedure seal; override; final;
  end;
implementation
procedure tbase.seal; begin end;
procedure tchild.seal; begin end;
end.
