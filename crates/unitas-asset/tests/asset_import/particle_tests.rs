use std::error::Error;

use unitas_asset::particle::{
    parse_particle_system, ParticleError, ParticleSystemBinding, PARTICLE_SYSTEM_CLASS,
};
use unitas_asset::unity_yaml::{self, Document};

type TestResult = Result<(), Box<dyn Error>>;

const DOCUMENT: &str = "--- !u!198 &1000\nParticleSystem:\n  m_Duration: 4\n  m_Loop: 1\n  m_MaxParticles: 64\n  m_StartLifetime: 2\n  m_StartSpeed: 3\n  m_StartSize: 0.5\n  m_StartColor:\n    r: 1\n    g: 0.5\n    b: 0.25\n    a: 0.5\n  m_EmissionRate: 12\n  m_Bursts:\n  - m_Time: 0.5\n    m_Count: 7\n    m_Cycles: 3\n";

fn document(text: &str) -> Result<Document, Box<dyn Error>> {
    let stream = unity_yaml::parse(text)?;
    let document = stream
        .documents
        .first()
        .cloned()
        .ok_or("the stream holds one document")?;
    Ok(document)
}

#[test]
fn a_particle_system_document_binds_every_contract_field() -> TestResult {
    let document = document(DOCUMENT)?;
    assert_eq!(document.class_id, PARTICLE_SYSTEM_CLASS);
    let binding = parse_particle_system(&document)?;
    assert_eq!(binding.duration, 4.0);
    assert!(binding.looping);
    assert_eq!(binding.max_particles, 64);
    assert_eq!(binding.start_lifetime, 2.0);
    assert_eq!(binding.start_speed, 3.0);
    assert_eq!(binding.start_size, 0.5);
    assert_eq!(binding.start_color, [1.0, 0.5, 0.25, 0.5]);
    assert_eq!(binding.emission_rate, 12.0);
    assert_eq!(binding.bursts.len(), 1);
    assert_eq!(binding.bursts[0].time, 0.5);
    assert_eq!(binding.bursts[0].count, 7);
    assert_eq!(binding.bursts[0].cycles, 3);
    Ok(())
}

#[test]
fn a_minimal_document_falls_back_to_the_default_binding() -> TestResult {
    let document = document("--- !u!198 &1\nParticleSystem:\n  m_Name: Empty\n")?;
    let binding = parse_particle_system(&document)?;
    assert_eq!(binding, ParticleSystemBinding::default());
    assert!(binding.bursts.is_empty());
    Ok(())
}

#[test]
fn a_document_of_another_class_is_rejected() -> TestResult {
    let document = document("--- !u!1 &1\nGameObject:\n  m_Name: Cube\n")?;
    assert_eq!(
        parse_particle_system(&document),
        Err(ParticleError::NotParticleSystem { class_id: 1 })
    );
    Ok(())
}
