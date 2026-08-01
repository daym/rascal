unit u;
interface
type
  plist = ^tlist;
  tlist = object
    first : plist;
    constructor init;
    destructor done; virtual;
    procedure add(item : integer); virtual;
  end;
implementation
end.
