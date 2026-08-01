unit useimpl;
interface
procedure compile;
implementation
uses dep;
type
  tolddata = record
    oldtoken : ttoken;
    oldpos : tfileposinfo;
    oldcall : tproccalloption;
  end;
procedure compile;
var
  data : tolddata;
begin
end;
end.
