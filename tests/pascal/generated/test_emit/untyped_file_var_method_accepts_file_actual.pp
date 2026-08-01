unit u;
interface
type
  tbuffer = object
    procedure blockwrite(var f : file);
  end;
procedure run;
implementation
procedure tbuffer.blockwrite(var f : file);
begin
end;
procedure run;
var
  arf : file;
  buffer : tbuffer;
begin
  buffer.blockwrite(arf);
end;
end.
