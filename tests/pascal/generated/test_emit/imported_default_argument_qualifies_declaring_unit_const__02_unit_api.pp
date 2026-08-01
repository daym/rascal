unit api;
interface
type
  thccflag = (hcc_check);
  thccflags = set of thccflag;
const
  hcc_all = [hcc_check];
procedure handle(flags : thccflags = hcc_all);
implementation
procedure handle(flags : thccflags);
begin
end;
end.
