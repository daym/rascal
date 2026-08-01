unit u;
interface
type
  tbase = class
    constructor Create;
    destructor Destroy; override;
    procedure m; virtual;
  end;
  tderived = class(tbase)
    procedure m; override;
  end;
implementation
end.
