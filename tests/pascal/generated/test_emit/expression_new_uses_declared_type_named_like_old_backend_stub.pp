unit u;
interface
type
  pimportlibwin32 = ^timportlibwin32;
  timportlibwin32 = object
    constructor init(n : integer);
  end;
function make : pimportlibwin32;
implementation
function make : pimportlibwin32;
begin
  make := new(pimportlibwin32, init(7));
end;
end.
