unit u;
interface
type
  uint32_t = longword;
  twriter = class
    procedure writeuint32(i : uint32_t);
    procedure run;
  end;
implementation
procedure twriter.writeuint32(i : uint32_t); begin end;
procedure twriter.run;
var
  m : longword;
begin
  writeuint32(ntole(m));
  writeuint32(ntobe(m));
end;
end.
