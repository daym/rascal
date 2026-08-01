unit u;
interface
type
  tbase = class
    procedure doit; virtual;
  end;
  tchild = class(tbase)
    procedure doit; reintroduce;
  end;
implementation
procedure tbase.doit; begin end;
procedure tchild.doit; begin end;
end.
