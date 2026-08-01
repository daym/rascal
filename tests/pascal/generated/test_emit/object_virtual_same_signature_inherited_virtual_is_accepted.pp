unit u;
interface
type
  tbase = object
    procedure doit; virtual;
  end;
  tchild = object(tbase)
    procedure doit; virtual;
  end;
implementation
procedure tbase.doit; begin end;
procedure tchild.doit; begin end;
end.
