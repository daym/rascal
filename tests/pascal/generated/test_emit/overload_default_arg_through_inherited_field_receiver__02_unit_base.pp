unit base;
interface
type
  topt = (oso_data, oso_keep);
  topts = set of topt;
  tkind = (sec_code, sec_data);
  tsection = class end;
  tobjdata = class
    function createsection(kind : tkind; name : shortstring = '') : tsection; overload;
    function createsection(name : shortstring; align : shortint;
      opts : topts; discard : boolean = true) : tsection; overload;
  end;
  texeoutput = class
  protected
    internaldata : tobjdata;
  end;
implementation
function tobjdata.createsection(kind : tkind; name : shortstring) : tsection;
begin createsection := nil; end;
function tobjdata.createsection(name : shortstring; align : shortint;
  opts : topts; discard : boolean) : tsection;
begin createsection := nil; end;
end.
