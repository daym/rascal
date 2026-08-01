unit ops;
interface
type
  tbox = record
    v : qword;
  end;
operator := (const n : qword) : tbox;
implementation
operator := (const n : qword) : tbox;
begin
  result.v := n;
end;
end.
