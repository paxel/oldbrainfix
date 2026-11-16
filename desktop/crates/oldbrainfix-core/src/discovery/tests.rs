use super::*;

#[test]
fn the_record_carries_the_device_id_and_nothing_else() -> Result<()> {
    let info = service_info("3f9a0c1d2e4b5a6c", 45231)?;
    assert_eq!(info.get_type(), SERVICE_TYPE);
    assert_eq!(
        info.get_fullname(),
        "3f9a0c1d2e4b5a6c._oldbrainfix._tcp.local."
    );
    assert_eq!(info.get_hostname(), "3f9a0c1d2e4b5a6c.local.");
    assert_eq!(info.get_port(), 45231);
    assert_eq!(info.get_properties().len(), 1);
    assert_eq!(info.get_property_val_str(ID_KEY), Some("3f9a0c1d2e4b5a6c"));
    assert!(info.get_addresses().is_empty(), "filled in as they come");
    Ok(())
}
