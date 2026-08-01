unit u;
interface
type
  tbase = class
    procedure seal; virtual; final;
  end;
  tchild = class(tbase)
    procedure childseal; virtual; final;
  end;
implementation
procedure tbase.seal; begin end;
procedure tchild.childseal; begin end;
end.
