unit u;
interface
type
  ttemp = class
  end;
  tdelete = class
    constructor create(t : ttemp);
  end;
procedure run(temp : ttemp);
implementation
constructor tdelete.create(t : ttemp);
begin
end;
procedure run(temp : ttemp);
begin
  tdelete.create(temp).free;
end;
end.
