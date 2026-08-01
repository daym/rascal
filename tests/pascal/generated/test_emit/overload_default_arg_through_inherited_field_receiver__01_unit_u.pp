unit u;
interface
uses base;
type
  tchild = class(texeoutput)
    procedure run;
  end;
implementation
procedure tchild.run;
var s : tsection;
begin
  s := internaldata.createsection('.reloc', 0, [oso_data, oso_keep]);
end;
end.
