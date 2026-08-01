unit u;
interface
procedure ext cdecl external 'libc' name 'ext';
type
  tfoo = class
    procedure a override;
    procedure b virtual abstract;
    procedure c virtual; abstract;
    procedure d; virtual abstract;
  end;
implementation
end.
