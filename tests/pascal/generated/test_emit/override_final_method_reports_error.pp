unit u;
interface
type
  tbase = class
    procedure seal; virtual; final;
  end;
  tchild = class(tbase)
    procedure seal; override;
  end;
implementation
procedure tbase.seal; begin end;
procedure tchild.seal; begin end;
end.
