unit u;
interface
type
  tnodeflag = (nf_one, nf_two);
  tnodeflags = set of tnodeflag;
procedure reset(arg : pointer);
implementation
procedure reset(arg : pointer);
var
  flags : tnodeflags;
begin
  flags := tnodeflags(arg^);
end;
end.
