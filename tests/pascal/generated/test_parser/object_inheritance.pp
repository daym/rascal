unit u;
interface
type
  tbase = object
    procedure m;
  end;
  tderived = object(tbase)
    procedure m; virtual;
  end;
implementation
end.
