unit u;
interface
type
  tsuperregister = type word;
  tcpuregisterset = set of 0..255;
procedure demo(registernumber : tsuperregister);
implementation
procedure demo(registernumber : tsuperregister);
var
  registers : tcpuregisterset;
begin
  registers := registers + [registernumber];
end;
end.
